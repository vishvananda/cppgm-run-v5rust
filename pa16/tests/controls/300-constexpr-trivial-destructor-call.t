// N3485 12.4: preserve the frozen interpreter's inert trivial-destructor call.
// expect exit 0
struct Value { int n; };
constexpr int evaluate() {
    Value value = { 3 };
    int result = value.n;
    value.~Value();
    Value().~Value();
    return result;
}
constexpr int result = evaluate();
static_assert(result == 3, "trivial calls retain receiver evaluation");
int main() { return result == 3 ? 0 : 1; }
