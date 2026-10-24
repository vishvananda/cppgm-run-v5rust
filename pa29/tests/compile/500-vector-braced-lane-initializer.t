typedef int words __attribute__((vector_size(16)));
inline words bad() { return words{{1},2,3,4}; }
