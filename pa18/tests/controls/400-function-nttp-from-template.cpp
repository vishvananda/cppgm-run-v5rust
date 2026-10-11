// N3485 14.3.2p1/p5, 13.4p1-p2: a function template name as the argument of a
// function-pointer non-type parameter selects the specialization matching the
// parameter type; expect exit 0
template<class T> T twice(T v) { return v + v; }
template<long (*F)(long)> long run(long v) { return F(v); }
int main() { return run<twice>(3) == 6 ? 0 : 1; }
