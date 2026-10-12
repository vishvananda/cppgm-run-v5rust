// Declaration name identity, callable facts and exception specification ownership.
namespace N{struct C{};int operator+(C,C);}namespace M{int N::operator+(C,C){return 1;}}
