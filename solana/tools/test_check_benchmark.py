import unittest
from check_benchmark import parse, validate

LOG = '''invalid=false CU=1000
invalid=true CU=1010
consumer valid CU=1100 headroom=1398900
consumer invalid CU=1090
upload chunks=8 bytes=6752 rent=48198000 largest_tx=1150'''
class BenchmarkTests(unittest.TestCase):
    def test_missing_verdict_is_not_a_benchmark(self):
        with self.assertRaises(ValueError): parse('build completed')
    def test_accepts_complete_measurement(self):
        values=parse(LOG);validate(values,values)
        self.assertEqual(values['consumer_valid_cu'],1100)
    def test_rejects_regression_and_runtime_limit(self):
        baseline=parse(LOG)
        for change in [{'valid_cu':1051},{'consumer_valid_cu':1400000},{'largest_upload_tx_bytes':1233}]:
            with self.assertRaises(ValueError): validate(baseline|change,baseline)
if __name__=='__main__': unittest.main()
