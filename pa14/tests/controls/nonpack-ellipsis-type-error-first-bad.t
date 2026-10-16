// expect reject
// The pattern's type error precedes the ellipsis's lack of a pack.
template<class T> void select(void&... values);
int main() { return 0; }
