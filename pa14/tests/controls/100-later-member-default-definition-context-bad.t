// N3485 14.6p10, 14.6.4.2: a later declaration's default remains invisible
// to an earlier template definition, even for a dependent call; expect reject
struct Item { int read(int); };
template<class T> int run(T item) { return item.read(); }
int Item::read(int n = 7) { return n; }
int main() { return run(Item()); }
