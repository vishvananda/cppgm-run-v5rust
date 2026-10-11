#pragma once

#include <cstdint>
#include <map>
#include <string>
#include <vector>

struct SourceLocation
{
  uint16_t file_index;
  uint32_t line;
  uint32_t column;
};

class SourceLocationTable
{
public:
  SourceLocationTable();

  uint16_t add_file(const std::string & path);
  uint32_t add(uint16_t file_index, uint32_t line, uint32_t column);
  std::string describe(uint32_t location_id) const;

  std::vector<std::string> files;
  std::vector<SourceLocation> locations;

private:
  std::map<std::string, uint16_t> file_indices_;
};

struct ISourceLocationProvider
{
  virtual ~ISourceLocationProvider() {}
  virtual const std::string & current_source_file() const = 0;
  virtual uint32_t current_source_line() const = 0;
  virtual uint32_t current_source_column() const = 0;
};
