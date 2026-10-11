// The condition of an explicit-specifier has to be a constant.
bool undecided();

struct broken {
  explicit(undecided()) broken(int);
};

int main() { return 0; }
