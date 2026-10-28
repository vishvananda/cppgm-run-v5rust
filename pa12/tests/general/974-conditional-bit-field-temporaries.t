int made;
int destroyed;
struct T { T() { ++made; } ~T() { ++destroyed; } };
struct A { unsigned value : 5; };
A object;
A& pick(const T&) { return object; }
void compound(bool condition) {
  (condition ? pick(T{}).value : pick(T{}).value) += 1;
}
void assign(bool condition) {
  (condition ? pick(T{}).value : pick(T{}).value) = 3;
}
void increment(bool condition) {
  ++(condition ? pick(T{}).value : pick(T{}).value);
}
void nested(bool first, bool second) {
  (first ? (second ? pick(T{}).value : pick(T{}).value) : pick(T{}).value) += 1;
}
bool check(const T&) { return false; }
void logical(bool first) {
  (first && check(T{}) ? pick(T{}).value : pick(T{}).value) = 7;
}
int main() {
  compound(true);
  compound(false);
  assign(false);
  increment(true);
  nested(true, false);
  nested(false, false);
  logical(true);
  return object.value == 7 && made == 8 && destroyed == 8 ? 0 : 1;
}
