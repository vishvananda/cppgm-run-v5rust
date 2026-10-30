// A braced class array is a full-expression while earlier objects are alive.
int destroyed;
int observed;
struct Guard { ~Guard() { ++destroyed; } };
struct Value { int n; static Value make(int v) { return Value{v}; } };
void use(const Value* p) { observed = p[0].n + p[1].n; }
void build(int n)
{
    Guard guard;
    const Value values[2] = {Value::make(n), Value::make(n + 1)};
    use(values);
}
int main() { build(3); return observed == 7 && destroyed == 1 ? 0 : 1; }
