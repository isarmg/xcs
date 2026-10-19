import json,unittest
from pathlib import Path
import numpy as np
import render_v3_animation as r
class AnimationChecks(unittest.TestCase):
    def test_duration_and_no_captions(self):
        story=json.loads((Path(__file__).parent/'story-v3.json').read_text())
        self.assertEqual(story['duration'],r.DURATION)
        for scene in story['scenes']:self.assertFalse(scene.get('render_caption',False))
    def test_whole_paper_content_does_not_change_shape(self):
        self.assertEqual(r.paper(8).size,r.paper(6).size)
        self.assertNotEqual(r.paper(8).tobytes(),r.paper(6).tobytes())
    def test_scene_boundaries_have_no_global_flash(self):
        for t in [9,18,27,36,48]:
            a=r.Canvas(640,360);b=r.Canvas(640,360)
            r.actors(a,t-1/30);r.actors(b,t)
            delta=np.abs(np.asarray(a.im,dtype=float)-np.asarray(b.im,dtype=float)).mean()
            self.assertLess(delta,8,msg=f'{t}: {delta}')
if __name__=='__main__':unittest.main()
