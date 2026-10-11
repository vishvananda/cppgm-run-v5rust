// Parser name facts are keyed by spelling, so a template template-parameter
// must not leave its own kind behind for a later parameter that reuses the
// name as an ordinary type.  Without that, the second _Sp stops starting a
// declarator and the parenthesis parses as a parameter clause.
template <class T, class U> struct impl {};

template <template <class, class...> class _Sp, class _Tp, class... _Args, class _Up>
struct impl<_Sp<_Tp, _Args...>, _Up> {};

template <class _Sp, class _Tp, class _Ap> class member_pointer_holder {
  _Sp (_Tp::*const_member)(_Ap) const;
  _Sp (_Tp::*plain_member)(_Ap);
};

template <template <class> class _Op> struct uses_template {};
template <class _Op, class _Cp> struct uses_type { _Op (_Cp::*member)(); };

int main() { return 0; }
