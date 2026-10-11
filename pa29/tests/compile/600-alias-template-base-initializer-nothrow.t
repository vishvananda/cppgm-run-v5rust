template<class T>
struct base
{
  base() noexcept {}
  base(const base&) noexcept {}
};

template<class T>
using base_alias = base<T>;

template<class T>
struct derived : base_alias<T>
{
  derived() noexcept {}
  derived(const derived& other) noexcept : base_alias<T>(other) {}
};

static_assert(__is_nothrow_constructible(derived<int>, const derived<int>&), "");

int main() { return 0; }
