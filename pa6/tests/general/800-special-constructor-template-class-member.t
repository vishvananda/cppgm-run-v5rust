// Frozen types-mode special-member declaration responsibility regression.
template<class T>struct C{template<class U>C(T p,U q){T a=p;U b=q;}};
