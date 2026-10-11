struct Box {
  int n;
  Box(const char *) : n(7) {}
};

Box pick(bool keep, Box b) {
  return keep ? b : "";
}
