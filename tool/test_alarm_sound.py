import pathlib
import tempfile
import unittest
import wave

from alarm_sound import write

COMMITTED = (
    pathlib.Path(__file__).resolve().parent.parent
    / "android/app/src/main/res/raw/obf_alarm.wav"
)


class AlarmSoundTest(unittest.TestCase):
    def test_the_committed_sound_is_what_the_script_writes(self):
        with tempfile.TemporaryDirectory() as tmp:
            fresh = pathlib.Path(tmp) / "obf_alarm.wav"
            write(fresh)
            self.assertEqual(fresh.read_bytes(), COMMITTED.read_bytes())

    def test_is_mono_16_bit_44100_hz_and_about_a_second(self):
        with wave.open(str(COMMITTED), "rb") as sound:
            self.assertEqual(sound.getnchannels(), 1)
            self.assertEqual(sound.getsampwidth(), 2)
            self.assertEqual(sound.getframerate(), 44100)
            seconds = sound.getnframes() / sound.getframerate()
            self.assertAlmostEqual(seconds, 1.2, places=2)


if __name__ == "__main__":
    unittest.main()
