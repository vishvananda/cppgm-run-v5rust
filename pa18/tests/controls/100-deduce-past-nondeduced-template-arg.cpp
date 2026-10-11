// N3485 14.8.2.5p4-p5: in MR<typename Str<C>::const_iterator, A> only the
// first argument is a non-deduced context; A is deduced from it (libstdc++
// regex_search(string, smatch, regex)); expect exit 0
template<class C> struct Str { typedef const C* const_iterator; };
template<class It, class A> struct MR {};
template<class C, class A> int srch(C, MR<typename Str<C>::const_iterator, A>) { return 0; }
int main() { MR<const char*, long> m; return srch('a', m); }
