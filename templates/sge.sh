#!/bin/bash
#$ -N {job_name}
#$ -q {queue}
#$ -pe {parallel_env} {cores}
#$ -cwd
module load {vasp_module}
cd {work_dir}
mpirun -np $NSLOTS {vasp_cmd}
