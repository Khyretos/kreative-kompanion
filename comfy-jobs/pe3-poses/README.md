# STU-PE3 pose example renders

Six pose editor presets rendered through SDXL with the union ControlNet (OpenPose 0.8, depth 0.45), 832 x 1216, one job at a time, 300 s timeout each. `jobs.json` lists each graph, its inputs (ComfyUI input name -> file here) and where the output goes. `inputs/*-parts.png` and `*-keypoints.json` are not used by the graph: the parts map is the new part-colour render, the keypoints are the target for the pose check.
