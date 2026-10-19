import unittest,json
from pathlib import Path
import numpy as np
import render_v4_branch as r
class BranchChecks(unittest.TestCase):
    def test_short_branch_scope(self):
        s=json.loads((Path(__file__).parent/'story-v4.json').read_text());self.assertEqual(s['duration'],10.5)
        for x in s['scenes']:self.assertFalse(x['render_caption'])
    def test_reordering_keeps_current_label_visible(self):
        import math
        for t in np.linspace(6,8.4,60):
            q=r.tween(t,6,8.4);rect=r.mix(r.MAIN,r.FINAL_MAIN,q)
            self.assertGreaterEqual(rect[1]-60*math.sin(math.pi*q)-52,0)
    def test_boundaries_no_global_flash(self):
        for t in [.65,2.55,3.25,4.25,6,8.4,8.8]:
            a=np.asarray(r.frame(t-1/30,640),dtype=float);b=np.asarray(r.frame(t,640),dtype=float)
            self.assertLess(np.abs(a-b).mean(),8,str(t))
if __name__=='__main__':unittest.main()
