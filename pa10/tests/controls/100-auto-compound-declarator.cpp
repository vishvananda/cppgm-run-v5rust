// N3485 7.1.6.4p6-p7: auto may appear under declarator operators (auto*&,
// const auto* const&); U is deduced per 14.8.2.1; expect exit 0
int y = 3;
int main() { int* p = &y; auto*& r = p; const auto* const& c = p; return *r == 3 && *c == 3 ? 0 : 1; }
