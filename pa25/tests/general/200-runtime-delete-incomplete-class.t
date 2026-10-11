// N3485 5.3.5p5: deleting an object of incomplete class type is well-formed
// (undefined only if the complete class has a non-trivial destructor or a
// deallocation function): no destructor runs and the usual deallocation
// function frees the storage, for delete and delete[] (also of an array of
// arrays)
