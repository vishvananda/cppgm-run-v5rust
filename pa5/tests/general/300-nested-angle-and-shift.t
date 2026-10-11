template<int n> struct leaf {};
template<class T> struct outer {};
outer<leaf<(6 >> 1)>> shifted;
