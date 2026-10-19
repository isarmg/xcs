"""Small deterministic preflight checks; no encoder or model download required."""
import json
import unittest
from pathlib import Path
import render

class RenderChecks(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.story=json.loads((Path(__file__).parent/'story-timed.json').read_text())
    def test_contiguous_timeline(self):
        end=0
        for scene in self.story['scenes']:
            self.assertEqual(scene['start'],end)
            self.assertGreater(scene['end'],scene['start'])
            end=scene['end']
        self.assertEqual(end,self.story['duration'])
    def test_text_capacity(self):
        for scene in self.story['scenes']:
            for step in scene['steps']:
                self.assertLessEqual(render.text_tile(step,17,'black',False).width,335,(scene['id'],step))
            for caption in render.caption_parts(scene['caption']):
                self.assertLessEqual(len(render.wrap(caption,94)),2,(scene['id'],caption))
    def test_speech_caption_timing(self):
        for scene in self.story['scenes']:
            self.assertTrue(scene.get('caption_timings'),scene['id'])
            previous=scene['start']
            for segment in scene['caption_timings']:
                self.assertGreaterEqual(segment['start'],previous)
                self.assertGreater(segment['end'],segment['start'])
                self.assertLessEqual(segment['end'],scene['end'])
                self.assertLessEqual(len(render.wrap(segment['text'],94)),2)
                previous=segment['end']
    def test_scene_frames(self):
        for scene in self.story['scenes']:
            frame=render.render(self.story,(scene['start']+scene['end'])/2,960,540)
            self.assertEqual(frame.size,(960,540))
            self.assertEqual(frame.mode,'RGB')
    def test_animation_changes(self):
        # Every shot has advancing progress/steps; graphical shots include motion paths.
        for scene in self.story['scenes']:
            a=render.render(self.story,scene['start']+1,640,360)
            b=render.render(self.story,scene['end']-1,640,360)
            self.assertNotEqual(a.tobytes(),b.tobytes(),scene['id'])
if __name__=='__main__': unittest.main()
