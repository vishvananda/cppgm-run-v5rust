struct Small {
  int first;
private:
  int empty[0];
public:
  int last;
  constexpr Small() : first(1), empty{}, last(2) {}
};
constexpr Small value{};
int main() { return 0; }
