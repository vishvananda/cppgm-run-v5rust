// N3485 14p4: a template shall not have C language linkage; expect reject
extern "C" { template<class T> struct S {}; }
int main() { return 0; }
