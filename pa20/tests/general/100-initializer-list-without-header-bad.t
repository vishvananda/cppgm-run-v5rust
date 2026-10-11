// N3485 8.5.4p2: std::initializer_list is not predefined; a range-for over {1,
// 2} without <initializer_list> is ill-formed; must be rejected
int main() { int s = 0; for (int b : {1, 2}) s += b; return s == 3 ? 0 : 1; }
