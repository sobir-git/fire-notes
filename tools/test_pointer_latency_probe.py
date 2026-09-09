import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from PIL import Image
import pointer_latency_probe as probe
import release


class PointerLatencyTests(unittest.TestCase):
    def test_locked_or_unknown_desktop_refuses_before_launch_and_artifacts(self):
        for state in ({'available': True, 'locked': True}, {'available': False}):
            with self.subTest(state=state), tempfile.TemporaryDirectory() as directory:
                output = Path(directory)/'uncreated'
                with patch.object(sys, 'argv', ['probe', '--physical-display', '--output', str(output)]), \
                     patch.object(probe, 'desktop_state', return_value=state), \
                     patch.object(probe.subprocess, 'Popen') as launch:
                    with self.assertRaisesRegex(RuntimeError, 'no app launched/input sent'):
                        probe.main()
                    launch.assert_not_called()
                    self.assertFalse(output.exists())

    def test_calibration_ignores_changes_outside_header_region(self):
        first = Image.new('RGB', (40, 40), 'black')
        second = first.copy()
        for x in range(40): second.putpixel((x, 30), (255, 0, 0))
        with self.assertRaisesRegex(AssertionError, 'eight changed'):
            probe.targets(first, second, (0, 0, 40, 10))
        for x in range(12): second.putpixel((x, 4), (255, 128, 0))
        states = probe.targets(first, second, (0, 0, 40, 10))
        self.assertEqual(len(states), 2)
        self.assertEqual(len(states[0]), 8)
        for before, after in zip(*states):
            self.assertEqual(before[:2], after[:2])
            self.assertEqual(before[2], [0, 0, 0])
            self.assertEqual(after[2], [255, 128, 0])
            self.assertEqual(after[1], 4)

    def test_virtual_display_requires_complete_observations(self):
        with self.assertRaises(Exception) as error:
            release.validate_latency({'schema': 1, 'display_kind': 'private_x11',
                                      'metric': 'native_input_to_drawable_pixels'}, Path('/tmp'), {})
        self.assertIn('did not complete', str(error.exception))

    def test_acknowledgement_metric_cannot_pass(self):
        with self.assertRaises(Exception) as error:
            release.validate_latency({'schema': 1, 'display_kind': 'physical_x11', 'metric': 'command_acknowledgement'}, Path('/tmp'), {})
        self.assertIn('not observed drawable response', str(error.exception))


if __name__ == '__main__':
    unittest.main()
