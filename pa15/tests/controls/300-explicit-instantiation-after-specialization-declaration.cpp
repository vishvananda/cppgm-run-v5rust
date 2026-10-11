// N3485 14.7.2p5: an explicit instantiation after the declaration of an
// explicit specialization (defined later) has no effect; expect exit 0
template<class T> int f(T) { return 1; }
template<> int f<int>(int);
template int f<int>(int);
template<> int f<int>(int) { return 3; }
int main() { return f(1) == 3 ? 0 : 1; }
