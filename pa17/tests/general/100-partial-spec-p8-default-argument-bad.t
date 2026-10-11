// N3485 14.5.5p8 (fourth bullet): a partial specialization template parameter
// list shall not contain default template arguments; expect reject
template<class T, class U> struct S {};
template<class T = int> struct S<T, T*> {};
int main() { return 0; }
