struct StringLike {
  StringLike(char const*) {}
};

int pick(StringLike const&, StringLike const&) { return 1; }
int pick(StringLike const&, char const*) { return 0; }

int main() {
  StringLike s("lhs");
  return pick(s, "rhs");
}
