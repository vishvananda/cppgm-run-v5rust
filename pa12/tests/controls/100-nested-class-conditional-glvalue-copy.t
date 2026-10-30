// A selected lvalue conditional supplies one source for the result's copy.
struct Value { int n; };
Value choose(bool outer, bool inner, Value a, const Value& b, const Value& c)
{
    return outer ? a : inner ? b : c;
}
int main()
{
    Value a{2};
    Value b{3};
    return choose(true, false, a, a, b).n == 2
        && choose(false, true, a, a, b).n == 2
        && choose(false, false, a, a, b).n == 3 ? 0 : 1;
}
