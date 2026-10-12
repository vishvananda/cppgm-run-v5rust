// Declaration name identity, callable facts and exception specification ownership.
namespace N{struct C{};int operator+(C,C);}int N::operator+(C a,C b){return 1;}
