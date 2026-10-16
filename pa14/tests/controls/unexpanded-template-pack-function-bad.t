// expect reject
// A non-expansion parameter type names a pack through a member-pointer owner.
template<class... Ts> void select(int Ts::*value);
int main() { return 0; }
