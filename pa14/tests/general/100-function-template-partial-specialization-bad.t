// N3485 14p2: in a function template declaration the last component of the
// declarator-id shall not be a template-id (no function template partial
// specialization); expect reject
template<class T> int f(T) { return 0; }
template<class T> int f<T*>(T*) { return 1; }
int main() { return 0; }
