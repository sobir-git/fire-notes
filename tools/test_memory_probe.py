"""Accounting checks that need neither a native app nor user data."""

import subprocess
import errno
import unittest
from unittest.mock import Mock, patch

import memory_probe as probe
from memory_accounting import drm_identity
from native_input import (
    KeyboardState,
    NativeInput,
    unshifted_ascii,
    validate_keyboard_state,
    chord_symbols,
)


def sample(dirty=1000, swap=0, swap_pss=0):
    return dict(Private_Dirty=dirty, Private_Clean=500, Swap=swap, SwapPss=swap_pss)


class AccountingTests(unittest.TestCase):
    def test_cpu_summary_keeps_lifetimes_and_transition_costs_separate(self):
        samples = [dict(launch=0, pid=12, t=t, cpu_ticks=ticks, stage=stage)
                   for t, ticks, stage in ((0, 10, 'typing'), (1, 20, 'typing'),
                                           (2, 50, 'selected'), (3, 70, 'selected'))]
        samples += [dict(launch=1, pid=13, t=t, cpu_ticks=ticks, stage='typing')
                    for t, ticks in ((4, 0), (5, 5))]
        result = probe.cpu_summary(samples, 100)
        self.assertAlmostEqual(result['sampled_process_cpu_seconds'], .65)
        self.assertEqual(result['observed_process_seconds'], 4)
        self.assertAlmostEqual(result['stages']['typing']['cpu_seconds'], .15)
        self.assertAlmostEqual(result['stages']['selected']['cpu_seconds'], .2)
        self.assertEqual(len(result['launches']), 2)

    def test_cpu_summary_rejects_decreasing_counter_and_reports_empty(self):
        self.assertFalse(probe.cpu_summary([], 100)['available'])
        samples = [dict(launch=0, pid=12, t=t, cpu_ticks=ticks, stage='typing')
                   for t, ticks in ((0, 10), (1, 5))]
        with self.assertRaisesRegex(ValueError, 'CPU sample'):
            probe.cpu_summary(samples, 100)

    def test_launch_environment_identifies_tuning_without_unrelated_secrets(self):
        env = {'GLIBC_TUNABLES': 'glibc.malloc.arena_max=1', 'MALLOC_TRIM_THRESHOLD_': '4096',
               'LD_PRELOAD': '', 'PRIVATE_API_KEY': 'not reportable', 'LIBGL_ALWAYS_SOFTWARE': '1'}
        result = probe.launch_environment(env)
        self.assertFalse(result['allocator_environment_is_default'])
        self.assertEqual(set(result['allocator_environment']), {'GLIBC_TUNABLES', 'MALLOC_TRIM_THRESHOLD_'})
        self.assertNotIn('PRIVATE_API_KEY', repr(result))
        self.assertTrue(probe.launch_environment({})['allocator_environment_is_default'])

    def test_desktop_lock_probe_reports_locked_and_unavailable(self):
        with patch.object(probe.subprocess, 'run', return_value=subprocess.CompletedProcess([],0,'(true,)','')) as run:
            self.assertTrue(probe.desktop_state({})['locked'])
            self.assertEqual(run.call_args.args[0][-1], 'org.gnome.ScreenSaver.GetActive')
        with patch.object(probe.subprocess, 'run', side_effect=FileNotFoundError('gdbus')):
            self.assertFalse(probe.desktop_state({})['available'])


    def test_native_chord_parser_accepts_workload_and_rejects_unknown_keys(self):
        for command in (
            "ctrl+Home",
            "ctrl+End",
            "ctrl+Tab",
            "ctrl+a",
            "ctrl+c",
            "ctrl+v",
            "ctrl+z",
            "ctrl+y",
            "ctrl+s",
            "ctrl+q",
            "Right",
        ):
            modifiers, key = chord_symbols(command)
            self.assertTrue(key)
            self.assertEqual(len(modifiers), int(command.startswith("ctrl+")))
        for command in ("ctrl+ctrl+q", "unknown+q", "ctrl+", "ctrl+שלום"):
            with self.assertRaises(ValueError):
                chord_symbols(command)

    def test_quit_chord_releases_key_and_modifier_before_server_ungrab(self):
        native = NativeInput.__new__(NativeInput)
        native.display = 1
        calls = []
        native.grab = lambda *_: calls.append("grab")
        native.ungrab = lambda *_: calls.append("ungrab")
        native.sync = lambda *_: calls.append("sync")
        native.guard = lambda _: KeyboardState()
        native.checked_keycode = lambda symbol, _: {0xFFE3: 37, ord("q"): 24}[symbol]

        def fake(_display, code, pressed, _delay):
            calls.append((code, pressed))
            return 1

        native.fake_key = fake
        evidence = native.chord(123, "ctrl+q")
        self.assertEqual(
            calls,
            ["grab", (37, 1), (24, 1), (24, 0), (37, 0), "sync", "ungrab", "sync"],
        )
        self.assertTrue(evidence["completed"])
        self.assertEqual(evidence["pressed_codes"], [37, 24])
        self.assertEqual(evidence["released_codes"], [24, 37])

    def test_partial_chord_failure_releases_only_successfully_pressed_keys(self):
        native = NativeInput.__new__(NativeInput)
        native.display = 1
        native.sync = lambda *_: None
        calls = []

        def fake(_display, code, pressed, _delay):
            calls.append((code, pressed))
            return int((code, pressed) != (24, 1))

        with self.assertRaises(RuntimeError):
            native.press_and_release(fake, [37, 24])
        self.assertEqual(calls, [(37, 1), (24, 1), (37, 0)])

    def test_release_failure_does_not_skip_remaining_owned_modifiers(self):
        native = NativeInput.__new__(NativeInput)
        native.display = 1
        native.sync = lambda *_: None
        calls = []

        def fake(_display, code, pressed, _delay):
            calls.append((code, pressed))
            return int((code, pressed) != (24, 0))

        with self.assertRaises(RuntimeError):
            native.press_and_release(fake, [37, 24])
        self.assertEqual(calls[-2:], [(24, 0), (37, 0)])

    def test_disappearing_proc_is_benign_only_after_expected_confirmed_exit_zero(self):
        vanished = ProcessLookupError(errno.ESRCH, "No such process")
        process = Mock()
        process.wait.return_value = 0
        self.assertTrue(probe.confirmed_clean_exit_race(vanished, process, True))
        self.assertFalse(probe.confirmed_clean_exit_race(vanished, process, False))
        process.wait.return_value = 1
        self.assertFalse(probe.confirmed_clean_exit_race(vanished, process, True))
        process.wait.side_effect = subprocess.TimeoutExpired("app", 0.1)
        self.assertFalse(probe.confirmed_clean_exit_race(vanished, process, True))
        process.reset_mock()
        self.assertFalse(
            probe.confirmed_clean_exit_race(
                PermissionError(errno.EACCES, "denied"), process, True
            )
        )
        process.wait.assert_not_called()

    def test_native_typing_never_rewrites_layout_for_unsupported_text(self):
        unshifted_ascii(probe.TYPED_TEXT)
        for text in ("Uppercase", "שלום", "\n"):
            with self.assertRaises(ValueError):
                unshifted_ascii(text)

    def test_native_input_refuses_held_keys_modifiers_and_pointer_buttons(self):
        validate_keyboard_state(KeyboardState(mods=16, locked_mods=16), 16, [0] * 32)
        for state, keys in [
            (KeyboardState(), [1]),
            (KeyboardState(mods=4), [0]),
            (KeyboardState(latched_mods=4), [0]),
            (KeyboardState(ptr_buttons=1), [0]),
        ]:
            with self.assertRaises(AssertionError):
                validate_keyboard_state(state, 16, keys)

    def test_native_focus_failure_releases_server_grab_without_injection(self):
        native = NativeInput.__new__(NativeInput)
        native.display = 1
        calls = []
        native.grab = lambda *_: calls.append("grab")
        native.ungrab = lambda *_: calls.append("ungrab")
        native.sync = lambda *_: calls.append("sync")

        def refuse(_window):
            raise AssertionError("Focus lost")

        native.guard = refuse
        with self.assertRaisesRegex(AssertionError, "Focus lost"):
            native.stroke(123, char="a")
        self.assertEqual(calls, ["grab", "ungrab", "sync"])

    def test_native_injection_records_partial_delivery_and_actual_elapsed(self):
        native = NativeInput.__new__(NativeInput)
        native.version = (2, 2)
        calls = []

        def stroke(_window, **args):
            if calls:
                raise AssertionError("Focus lost")
            calls.append(args)

        native.stroke = stroke
        evidence = {}
        with self.assertRaises(AssertionError):
            native.send(123, text="ab", delay=0, evidence=evidence)
        self.assertEqual(evidence["requested_events"], 2)
        self.assertEqual(evidence["delivered_events"], 1)
        self.assertGreaterEqual(evidence["elapsed_seconds"], 0)
        self.assertEqual(len(evidence["event_timings"]), 1)

    def test_logical_window_workload_stays_large_at_hidpi(self):
        scale = probe.infer_scale((1200, 800), (600, 400))
        self.assertEqual(scale, 2)
        self.assertEqual(
            [probe.physical_size(size, scale) for size in probe.LOGICAL_WINDOW_SIZES],
            [(1200, 800), (1800, 1200), (2400, 1600)],
        )
        with self.assertRaises(ValueError):
            probe.infer_scale((1200, 600), (600, 400))

    def test_document_check_rejects_partial_typing_and_unicode_normalization(self):
        expected = (probe.TYPED_TEXT + "e\u0301 שלום").encode()
        self.assertTrue(probe.document_evidence(expected, expected)["equal"])
        self.assertFalse(probe.document_evidence(expected[:30], expected)["equal"])
        normalized = (probe.TYPED_TEXT + "é שלום").encode()
        self.assertFalse(probe.document_evidence(normalized, expected)["equal"])

    def test_missing_native_outcome_times_out_instead_of_passing(self):
        with patch.object(probe.time, "monotonic", side_effect=[0, 0, 6]), patch.object(
            probe.time, "sleep"
        ):
            with self.assertRaisesRegex(AssertionError, "not saved"):
                probe.wait_until(lambda: False, "not saved")

    def test_exact_exclusive_limit_at_proc_kib_granularity(self):
        below = probe.memory_budget([sample(2929)])
        above = probe.memory_budget([sample(2930)])
        self.assertEqual(below["target_bytes_exclusive"], 3_000_000)
        self.assertEqual(below["limit_bytes_exclusive"], 4_000_000)
        self.assertEqual(below["max_sampled_bytes"], 2_999_296)
        self.assertTrue(below["passed"])
        self.assertEqual(above["max_sampled_bytes"], 3_000_320)
        self.assertTrue(above["passed"])
        self.assertTrue(below["target_met"])
        self.assertFalse(above["target_met"])
        self.assertTrue(probe.memory_budget([sample(3906)])["passed"])
        self.assertFalse(probe.memory_budget([sample(3907)])["passed"])

    def test_total_private_is_reported_separately(self):
        result = probe.memory_budget([sample(2929)])
        self.assertTrue(result["passed"])
        self.assertEqual(
            result["max_total_private_resident_bytes"], (2929 + 500) * 1024
        )

    def test_swap_in_any_sample_rejects_apparent_savings(self):
        self.assertFalse(probe.memory_budget([sample(), sample(swap=1)])["passed"])
        self.assertFalse(probe.memory_budget([sample(), sample(swap_pss=1)])["passed"])

    def test_empty_invalid_or_unreadable_samples_fail_closed(self):
        for values in [[], [{}], [sample(-1)], [sample(1.5)]]:
            self.assertFalse(probe.memory_budget(values)["passed"])
        self.assertFalse(
            probe.memory_budget([sample()], [{"error": "permission denied"}])["passed"]
        )

    def test_duplicate_drm_fds_share_identity_but_devices_do_not(self):
        fields = {"drm-pdev": "0000:00:02.0", "drm-client-id": "123"}
        self.assertEqual(drm_identity(fields, 14), drm_identity(fields, 15))
        self.assertNotEqual(
            drm_identity(fields, 14),
            drm_identity({**fields, "drm-pdev": "0000:01:00.0"}, 14),
        )
        self.assertNotEqual(drm_identity({}, 14), drm_identity({}, 15))

    @patch.object(probe, "display_processes", return_value=[1])
    @patch.object(
        probe.subprocess, "run", side_effect=subprocess.TimeoutExpired("observer", 15)
    )
    def test_external_timeout_is_unavailable_not_zero(self, _run, _processes):
        result = probe.external_costs({"DISPLAY": ":0"}, "typing", 2)
        self.assertFalse(result["available"])
        self.assertIn("error", result)

    @patch.object(probe, "display_processes", return_value=[1])
    def test_missing_app_xres_client_cannot_claim_attribution(self, _processes):
        import json

        data = dict(processes=[], xres_clients=[], query_errors=[])
        with patch.object(
            probe.subprocess,
            "run",
            return_value=subprocess.CompletedProcess([], 0, json.dumps(data), ""),
        ):
            result = probe.external_costs({"DISPLAY": ":0"}, "typing", 2)
        self.assertFalse(result["available"])
        self.assertEqual(result["app_xres_client_count"], 0)


if __name__ == "__main__":
    unittest.main()
