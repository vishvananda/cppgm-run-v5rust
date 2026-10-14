struct Alias { int& target; };
struct Record { Alias aliases[2]; };
Record value = {};
