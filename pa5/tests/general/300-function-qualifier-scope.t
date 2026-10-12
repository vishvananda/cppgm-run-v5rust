struct C{};
int f(int C) noexcept(sizeof(C));
int ((g))(int C){sizeof(C);}
namespace N{typedef int word; struct C{};}
int (N::h)(word value) noexcept(sizeof(C));
int (N::values)[sizeof(word)][sizeof(word)];
