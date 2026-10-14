// expect reject
// A class's hidden friend is found by ADL, but never by ordinary lookup.
struct Box {
  friend int inspect(Box &);
};
int (*address)(Box &) = inspect;
int main() { return 0; }
