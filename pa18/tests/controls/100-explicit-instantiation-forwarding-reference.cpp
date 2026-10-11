// N3485 14.7.2p8, 14.8.2.6, 14.8.2.5, 8.3.2p6: `template int one(long&);`
// deduces T = long& for one(T&&) (long& && collapses to long&); expect exit 0
template <class T> int one(T&&) { return 1; }
template int one(long&);
int main() { long i = 0; return one(i) == 1 ? 0 : 1; }
