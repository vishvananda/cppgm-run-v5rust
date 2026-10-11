// N3485 14.5.5p8, 14.5.3p5: a partial specialization argument shall not
// contain an unexpanded parameter pack; expect reject
template<class T> struct L {};
template<class... Ts> struct L<Ts> {};
int main() { return 0; }
