// A synthesized copy or move constructor of a class with virtual bases: the
// complete object's constructor copies the virtual bases first; when a later
// copy throws, each copied base subobject that has virtual bases is destroyed
// by its base-object destructor, which leaves the shared virtual bases to the
// complete object's cleanup (12.4p8, 15.2p2: each subobject is destroyed
// once). Every copy ticks a countdown that throws at step k; each shape sweeps
// k and checks no object stays alive or dies twice
