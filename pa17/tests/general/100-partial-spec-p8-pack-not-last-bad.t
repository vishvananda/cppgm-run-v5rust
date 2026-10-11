// N3485 14.5.5p8 (last bullet): a pack expansion argument of a partial
// specialization shall be the last argument; expect reject
template<class... Ts> struct L {};
template<class... Ts> struct L<Ts..., int> {};
int main() { return 0; }
