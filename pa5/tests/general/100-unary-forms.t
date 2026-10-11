template<class... Types>
void unary(int x) {
  --x;
  +x;
  -x;
  !x;
  sizeof !x;
  sizeof...(Types);
}
