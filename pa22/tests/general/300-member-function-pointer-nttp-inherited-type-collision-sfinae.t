// VALIDATION: compile-pass
// The Boost-style member detector forms &U::name as a member-function-pointer
// non-type template argument. If another base contributes a type with that
// name, the expression is ill-formed during substitution and the ellipsis
// overload is selected.

typedef char yes_type;
typedef char no_type[2];

template <typename Type>
struct has_member_named_hasher
{
   struct BaseMixin
   {
      void hasher();
   };
   struct Base : public Type, public BaseMixin
   {
      Base();
   };
   template <typename T, T t> class Helper {};
   template <typename U>
   static no_type & test(U*, Helper<void (BaseMixin::*)(), &U::hasher>* = 0);
   static yes_type & test(...);
   static const bool value = sizeof(yes_type) == sizeof(test((Base*)0));
};

struct unordered_like
{
   typedef int hasher;
};

struct ordered_like
{
};

static_assert(has_member_named_hasher<unordered_like>::value,
              "nested type member should make the detector select yes");
static_assert(!has_member_named_hasher<ordered_like>::value,
              "inherited BaseMixin function alone should select no");

int main()
{
   return has_member_named_hasher<unordered_like>::value &&
          !has_member_named_hasher<ordered_like>::value ? 0 : 1;
}
