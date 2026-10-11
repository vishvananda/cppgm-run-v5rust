// VALIDATION: compile-pass
// std::nullptr_t has built-in equality and inequality comparisons with itself.

typedef decltype(nullptr) nullptr_t;

bool equal_refs(const nullptr_t & a, const nullptr_t & b)
{
   return a == b;
}

bool not_equal_values()
{
   return nullptr != nullptr;
}

int main()
{
   nullptr_t a = nullptr;
   nullptr_t b = nullptr;
   return equal_refs(a, b) && !not_equal_values() ? 0 : 1;
}
