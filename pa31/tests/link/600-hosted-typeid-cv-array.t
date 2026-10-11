// N3485 5.2.8p5 + 3.9.3p5: an array of const int is a const-qualified type, so
// typeid(const int[3]) == typeid(int[3]) (top-level cv ignored)
