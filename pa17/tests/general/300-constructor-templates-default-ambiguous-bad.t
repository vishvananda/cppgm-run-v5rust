// N3485 13.3.3p1-p2, 14.5.6.2p3-p5: two default-constructor templates whose
// template parameters all have defaults are not ordered by a call without
// arguments; K v; is ambiguous; must be rejected
struct K { template<class T = int> K() {} template<class T = long, class U = int> K() {} };
int main() { K v; (void)v; return 0; }
