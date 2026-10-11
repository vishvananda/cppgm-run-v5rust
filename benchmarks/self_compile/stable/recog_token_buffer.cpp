#include <stdexcept>

using namespace std;

#include "recog_token_buffer.h"

#ifdef CPPGM_DEBUG_RECOG_BUFFER
#include <cstdlib>
#include <fstream>
#endif

namespace {

string token_text_for_span(const RecogToken & token)
{
  if(token.is_rshift_piece()) {
    return ">";
  }
  return token.source;
}

}  // namespace

RecogTokenBuffer::RecogTokenBuffer(IRecogTokenSource & source,
                                   const string & debug_label,
                                   const SourceLocationTable * locations) :
  source(source),
  finished(false),
  locations(locations)
#ifdef CPPGM_DEBUG_RECOG_BUFFER
  ,
  debug_label(debug_label),
  debug_max_materialized_tokens(0),
  debug_max_peek_offset(0),
  debug_max_buffered_ahead(0),
  debug_forced_full_materializations(0),
  debug_max_forced_materialized_tokens(0),
  debug_used(false)
#endif
{
#ifndef CPPGM_DEBUG_RECOG_BUFFER
  (void)debug_label;
#endif
}

RecogTokenBuffer::~RecogTokenBuffer()
{
#ifdef CPPGM_DEBUG_RECOG_BUFFER
  const char * path = getenv("CPPGM_RECOG_BUFFER_STATS_FILE");
  if(!path || !*path || !debug_used) {
    return;
  }

  ofstream out(path, ios::app);
  out << "label=" << (debug_label.empty() ? "<unknown>" : debug_label)
      << " max_materialized_tokens=" << debug_max_materialized_tokens
      << " max_peek_offset=" << debug_max_peek_offset
      << " max_buffered_ahead=" << debug_max_buffered_ahead
      << " forced_full_materializations=" << debug_forced_full_materializations
      << " max_forced_materialized_tokens=" << debug_max_forced_materialized_tokens
      << '\n';
#endif
}

const RecogToken & RecogTokenBuffer::operator[](size_t index) const
{
  ensure(index);
  if(index >= tokens.size()) {
    throw logic_error("recog token index out of range");
  }
  return tokens[index];
}

const RecogToken & RecogTokenBuffer::peek(size_t index) const
{
  ensure(index);
  if(index < tokens.size()) {
    return tokens[index];
  }
  return last_pulled();
}

size_t RecogTokenBuffer::size() const
{
#ifdef CPPGM_DEBUG_RECOG_BUFFER
  bool was_finished = finished;
#endif
  materialize_all();
#ifdef CPPGM_DEBUG_RECOG_BUFFER
  if(!was_finished) {
    debug_used = true;
    ++debug_forced_full_materializations;
    if(tokens.size() > debug_max_forced_materialized_tokens) {
      debug_max_forced_materialized_tokens = tokens.size();
    }
  }
#endif
  return tokens.size();
}

const RecogToken & RecogTokenBuffer::back() const
{
#ifdef CPPGM_DEBUG_RECOG_BUFFER
  bool was_finished = finished;
#endif
  materialize_all();
#ifdef CPPGM_DEBUG_RECOG_BUFFER
  if(!was_finished) {
    debug_used = true;
    ++debug_forced_full_materializations;
    if(tokens.size() > debug_max_forced_materialized_tokens) {
      debug_max_forced_materialized_tokens = tokens.size();
    }
  }
#endif
  return tokens.back();
}

vector<RecogToken> RecogTokenBuffer::slice(size_t start, size_t end) const
{
  if(end < start) {
    throw logic_error("invalid token slice");
  }

  if(end != 0) {
    ensure(end - 1);
  }
  return vector<RecogToken>(tokens.begin() + start, tokens.begin() + end);
}

string RecogTokenBuffer::span_text(size_t start, size_t end) const
{
  if(end < start) {
    throw logic_error("invalid token span");
  }

  if(end != 0) {
    ensure(end - 1);
  }

  string text;
  for(size_t i = start; i < end; ++i) {
    text += token_text_for_span(tokens[i]);
  }
  return text;
}

const SourceLocationTable * RecogTokenBuffer::source_locations() const
{
  return locations;
}

#ifdef CPPGM_DEBUG_RECOG_BUFFER
void RecogTokenBuffer::debug_note_peek(size_t pos, size_t offset) const
{
  debug_used = true;
  if(offset > debug_max_peek_offset) {
    debug_max_peek_offset = offset;
  }

  size_t buffered_ahead = 0;
  if(tokens.size() > pos + 1) {
    buffered_ahead = tokens.size() - pos - 1;
  }
  if(buffered_ahead > debug_max_buffered_ahead) {
    debug_max_buffered_ahead = buffered_ahead;
  }
}
#endif

void RecogTokenBuffer::ensure(size_t index) const
{
  while(!finished && tokens.size() <= index) {
    RecogToken token = source.get();
    tokens.push_back(token);
#ifdef CPPGM_DEBUG_RECOG_BUFFER
    debug_used = true;
    if(tokens.size() > debug_max_materialized_tokens) {
      debug_max_materialized_tokens = tokens.size();
    }
#endif
    if(token.is_eof() || token.is_invalid()) {
      finished = true;
    }
  }

  if(tokens.empty()) {
    throw logic_error("empty recog token buffer");
  }
}

void RecogTokenBuffer::materialize_all() const
{
  while(!finished) {
    RecogToken token = source.get();
    tokens.push_back(token);
#ifdef CPPGM_DEBUG_RECOG_BUFFER
    debug_used = true;
    if(tokens.size() > debug_max_materialized_tokens) {
      debug_max_materialized_tokens = tokens.size();
    }
#endif
    if(token.is_eof() || token.is_invalid()) {
      finished = true;
    }
  }

  if(tokens.empty()) {
    throw logic_error("empty recog token buffer");
  }
}

const RecogToken & RecogTokenBuffer::last_pulled() const
{
  if(tokens.empty()) {
    throw logic_error("empty recog token buffer");
  }
  return tokens.back();
}
