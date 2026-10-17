// expect reject
// A named definition requires the selected member's definition at unit end.
template<class T> struct box { ~box(); };
template box<int>::~box();
int main() { return 0; }
