// N3485 2.14.8 and 5.19: retain the frozen evaluator's literal glvalue boundary.
// expect reject
constexpr const char& operator""_reference(const char* p, unsigned long) { return p[1]; }
constexpr auto& reference = "AB"_reference;
