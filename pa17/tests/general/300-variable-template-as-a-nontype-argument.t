// The parser reads a template-id template-argument as a type-id, which is the
// right guess for most of them and wrong for a variable template.  Failing to
// build it as a type is how that is discovered, so the explicit-argument path
// has to report it the way it already reports any other non-type argument --
// by falling back to the value path -- rather than treating the failure as an
// error.
//
// libc++ writes `__is_invocable_r_impl<_Ret, __is_invocable_v<_Args...>, _Args...>`,
// whose middle argument is a bool spelled as a variable template over a pack.

template<class... A>
const bool invocable_v = true;

template<class R, bool B, class... A>
const bool impl_v = B;

template<class R, class... A>
const bool r_v = impl_v<R, invocable_v<A...>, A...>;

// The same shape without a pack, so the fix is not specific to one.
template<class T>
const bool plain_v = true;

template<bool B>
const bool takes_bool_v = B;

template<class T>
const bool through_v = takes_bool_v<plain_v<T> >;

int main()
{
  int status = 0;
  if (!r_v<int, char>) status = 1;
  if (!through_v<char>) status = 2;
  return status;
}
