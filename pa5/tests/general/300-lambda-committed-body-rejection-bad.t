struct C {};
void parse_invalid() {
  C object(int([] { return 1 + ; }()));
}
