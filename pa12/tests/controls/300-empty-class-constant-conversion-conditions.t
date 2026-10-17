// N3485 5.16, 12.3: empty-class condition conversion retains effects of
// nontrivial construction and destruction; expect exit 0
int effects;
struct Constant {
  static const bool value = true;
  operator bool() const { return value; }
};
struct Constructed : Constant { Constructed() { ++effects; } };
struct Destroyed : Constant { ~Destroyed() { ++effects; } };
int main() {
  Constant constant;
  int result = constant ? 0 : 1;
  result += Constant{} ? 0 : 2;
  result += Constructed() ? 0 : 4;
  result += Destroyed() ? 0 : 8;
  return result + (effects == 2 ? 0 : 16);
}
