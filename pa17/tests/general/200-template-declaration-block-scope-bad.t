// N3485 14p2: a template-declaration can appear only as a namespace scope or
// class scope declaration; expect reject
int main() { template<class T> using A = T; return 0; }
