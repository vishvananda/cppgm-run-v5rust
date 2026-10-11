// N3485 14.1p12: a template-parameter shall not be given default arguments by
// two different declarations; expect reject
template<class T = int> struct S;
template<class T = int> struct S {};
int main() { return 0; }
