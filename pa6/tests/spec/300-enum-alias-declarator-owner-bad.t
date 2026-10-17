// N3485 focus: 8.3 [dcl.meaning] a qualified declarator-id declares a
// namespace or class member; an enum's lookup scope is not a declaration home.
enum class Enumeration { value };
typedef Enumeration Alias;
int Alias::member();
