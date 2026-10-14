// N3485 14.5.6.1p4, 1.3.17, 13.4p2: function templates differing only in
// return type are distinct templates; the target type selects k<int>;
// expect exit 0
template<class T> int k(T);
template<class T> long k(T);
template<class T> int k(T) { return 5; }
template<class T> long k(T) { return 6; }
int main() { int (*ik)(int) = k<int>; long (*lk)(int) = k<int>; return (ik(0) == 5 && lk(0) == 6) ? 0 : 1; }
