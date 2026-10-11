// N3485 5.1.2p23, 14.5.3p5: a capture followed by an ellipsis must name a
// parameter pack; [x...] with a non-pack x is ill-formed. Must reject.
template<class T> int f(T x) { return [x...] { return x; }(); }
int main() { return f(1); }
