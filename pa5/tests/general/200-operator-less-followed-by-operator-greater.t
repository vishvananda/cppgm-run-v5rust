// HHC-131
template<class A, class B> struct pair { A first; B second; };

template <class _T1, class _T2, class _U1, class _U2>
inline bool operator<(const pair<_T1, _T2>& __x, const pair<_U1, _U2>& __y) {
  return __x.first < __y.first;
}

template <class _T1, class _T2, class _U1, class _U2>
inline bool operator>(const pair<_T1, _T2>& __x, const pair<_U1, _U2>& __y) {
  return __y < __x;
}
