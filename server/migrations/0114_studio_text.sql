-- KS-01: audio runs stored params and models as BLOBs, which the run and list reads cannot decode.
UPDATE studio_run SET params = CAST(params AS TEXT) WHERE typeof(params) = 'blob';
UPDATE studio_run SET models = CAST(models AS TEXT) WHERE typeof(models) = 'blob';
UPDATE studio_run SET outputs = CAST(outputs AS TEXT) WHERE typeof(outputs) = 'blob';
