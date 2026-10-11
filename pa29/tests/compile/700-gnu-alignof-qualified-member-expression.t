template<class T> struct buffer {
  struct member { T value; };
  alignas(__alignof__(member::value)) char data;
};
buffer<long> value;

using alias_int __attribute__((aligned(1))) = int;
typedef int typedef_int __attribute__((aligned(1)));
alias_int alias_value;
typedef_int typedef_value;
static_assert(__alignof__(alias_value) == 1, "using alias alignment");
static_assert(__alignof__(typedef_value) == 1, "typedef alignment");
static_assert(__alignof__(*(&typedef_value)) == 1, "indirect alignment");
static_assert(__is_same(alias_int, int), "canonical type identity");
static_assert(__alignof__(int) == 4, "ordinary type alignment");

struct alias_member_layout {
  char first;
  alias_int second;
};
static_assert(sizeof(alias_member_layout) == 5, "member layout");
static_assert(__builtin_offsetof(alias_member_layout, second) == 1,
              "member offset");
