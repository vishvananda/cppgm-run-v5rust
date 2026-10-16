// N3485 14.7.2: an explicit definition checks its body without a call; expect reject
template<class T> int unused(T value) { return T::missing; }
template int unused(int);
int main() { return 0; }
