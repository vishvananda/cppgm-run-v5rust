namespace boost {
template<class T> struct remove_all_extents { typedef T type; };
template<class T> using remove_all_extents_t = typename remove_all_extents<T>::type;
}
char pick(const bool*);
long pick(bool*);
static_assert(sizeof(pick((boost::remove_all_extents_t<const bool>*)0)) == sizeof(char), "");
