#include <nlohmann/json.hpp>
#include "internal_highlight.h"
#include "util.h"

#ifdef SWEETLINE_DEBUG
#define DUMP_JSON_TO_RESULT(json, result) result = json.dump(2);
#else
#define DUMP_JSON_TO_RESULT(json, result) result = json.dump();
#endif

namespace NS_SWEETLINE
{
  // ===================================== TokenSpan ============================================
  bool TokenSpan::operator==(const TokenSpan &other) const
  {
    return range == other.range && style_id == other.style_id && state == other.state && goto_state == other.goto_state;
  }

  bool TokenSpan::operator!=(const TokenSpan &other) const
  {
    return !this->operator==(other);
  }

#ifdef SWEETLINE_DEBUG
  void TokenSpan::dump() const
  {
    const nlohmann::json json = *this;
    std::cout << json.dump(2) << std::endl;
  }
#endif

  // ===================================== LineHighlight ============================================
  void LineHighlight::pushOrMergeSpan(TokenSpan &&span)
  {
    if (spans.empty())
    {
      spans.push_back(std::move(span));
    }
    else
    {
      TokenSpan &last = spans.back();
      if (last.range.end.column == span.range.start.column && last.style_id == span.style_id)
      {
        last.range.end.column = span.range.end.column;
        last.range.end.index = span.range.end.index;
      }
      else
      {
        spans.push_back(std::move(span));
      }
    }
  }

  bool LineHighlight::operator==(const LineHighlight &other) const
  {
    if (spans.size() != other.spans.size())
    {
      return false;
    }
    const size_t size = spans.size();
    for (size_t i = 0; i < size; ++i)
    {
      if (spans[i] != other.spans[i])
      {
        return false;
      }
    }
    return true;
  }

  void LineHighlight::toJson(U8String &result) const
  {
    nlohmann::json json = *this;
    DUMP_JSON_TO_RESULT(json, result);
  }

#ifdef SWEETLINE_DEBUG
  void LineHighlight::dump() const
  {
    const nlohmann::json json = *this;
    std::cout << json.dump(2) << std::endl;
  }
#endif

  // ===================================== DocumentHighlight ============================================
  void DocumentHighlight::addLine(LineHighlight &&line)
  {
    lines.push_back(std::move(line));
  }

  size_t DocumentHighlight::spanCount() const
  {
    size_t count = 0;
    for (const LineHighlight &line : lines)
    {
      count += line.spans.size();
    }
    return count;
  }

  void DocumentHighlight::reset()
  {
    lines.clear();
  }

  void DocumentHighlight::toJson(U8String &result) const
  {
    nlohmann::json json = *this;
    DUMP_JSON_TO_RESULT(json, result);
  }

#ifdef SWEETLINE_DEBUG
  void DocumentHighlight::dump() const
  {
    nlohmann::json json = *this;
    std::cout << json.dump(2) << std::endl;
  }
#endif

  // ===================================== LineState ============================================
#ifdef SWEETLINE_DEBUG
  void CodeBlock::dump() const
  {
    nlohmann::json json = *this;
    std::cout << json.dump(2) << std::endl;
  }
#endif

  // ===================================== LineState ============================================
  bool LineBlockState::operator==(const LineBlockState &other) const
  {
    return nesting_level == other.nesting_level && block_state == other.block_state && block_column == other.block_column;
  }

  // ===================================== HighlightConfig ============================================
  HighlightConfig HighlightConfig::kDefault = {};

  // ===================================== HighlightConfig ============================================
  TextAnalyzer::TextAnalyzer(const SharedPtr<SyntaxRule> &rule, const HighlightConfig &config)
  {
    m_line_highlight_analyzer_ = makeUniquePtr<LineHighlightAnalyzer>(rule, config);
  }

  SharedPtr<DocumentHighlight> TextAnalyzer::analyzeText(const U8String &text)
  {
    auto highlight = std::make_shared<DocumentHighlight>();
    if (text.empty())
    {
      return highlight;
    }

    TextLineInfo line_info;
    line_info.line = 0;
    line_info.start_state = SyntaxRule::kDefaultStateId;
    line_info.start_char_offset = 0;

    size_t start = 0;
    size_t end = text.find('\n');

    while (end != U8String::npos)
    {
      size_t len = end - start;
      U8String line_text = text.substr(start, len);

      bool has_cr = (!line_text.empty() && line_text.back() == '\r');
      if (has_cr)
      {
        line_text.pop_back();
      }

      LineAnalyzeResult result;
      analyzeLine(line_text, line_info, result);
      highlight->addLine(std::move(result.highlight));

      line_info.line++;
      line_info.start_state = result.end_state;
      line_info.start_char_offset += result.char_count + (has_cr ? 1 : 0) + 1;

      start = end + 1;
      end = text.find('\n', start);
    }

    // Last line
    U8String last_line = text.substr(start);
    bool has_cr = (!last_line.empty() && last_line.back() == '\r');
    if (has_cr)
    {
      last_line.pop_back();
    }

    LineAnalyzeResult result;
    analyzeLine(last_line, line_info, result);
    highlight->addLine(std::move(result.highlight));

    return highlight;
  }

  void TextAnalyzer::analyzeLine(const U8String &text, const TextLineInfo &line_info, LineAnalyzeResult &result) const
  {
    m_line_highlight_analyzer_->analyzeLine(text, line_info, result);
  }

  const HighlightConfig &TextAnalyzer::getHighlightConfig() const
  {
    return m_line_highlight_analyzer_->getHighlightConfig();
  }

  // ===================================== LineHighlightAnalyzer ============================================
  LineHighlightAnalyzer::LineHighlightAnalyzer(const SharedPtr<SyntaxRule> &syntax_rule, const HighlightConfig &config)
      : m_rule_(syntax_rule), m_config_(config)
  {
  }

  void LineHighlightAnalyzer::analyzeLine(const U8String &text, const TextLineInfo &info, LineAnalyzeResult &result) const
  {
    if (text.empty())
    {
      result.end_state = info.start_state;
      result.char_count = 0;
      return;
    }

    size_t current_char_pos = 0;
    int32_t current_state = info.start_state;
    size_t line_char_count = Utf8Util::countChars(text);
    // 一直匹配到当前行最后一个字符
    while (current_char_pos < line_char_count)
    {
      MatchResult match_result = matchAtPosition(text, current_char_pos, current_state);
      if (!match_result.matched)
      {
        current_char_pos++;
        continue;
      }
      addLineHighlightResult(result.highlight, info, current_state, match_result);
      current_char_pos = match_result.start + match_result.length;
      if (match_result.goto_state >= 0)
      {
        current_state = match_result.goto_state;
      }
    }
    StateRule &state_rule = m_rule_->getStateRule(current_state);
    if (state_rule.line_end_state >= 0)
    { // 如果当前状态有行结束状态，则将当前状态切换到行结束状态
      current_state = state_rule.line_end_state;
    }
    result.end_state = current_state;
    result.char_count = line_char_count;
  }

  const HighlightConfig &LineHighlightAnalyzer::getHighlightConfig() const
  {
    return m_config_;
  }

  MatchResult LineHighlightAnalyzer::matchAtPosition(const U8String &text, size_t start_char_pos, int32_t syntax_state) const
  {
    MatchResult result;
    if (!m_rule_->containsRule(syntax_state))
    {
      return result;
    }
    const StateRule &state_rule = m_rule_->getStateRule(syntax_state);
    return matchAtPosition(text, start_char_pos, state_rule);
  }

  MatchResult LineHighlightAnalyzer::matchAtPosition(const U8String &text, size_t start_char_pos, const StateRule &state_rule) const
  {
    MatchResult result;
    size_t start_byte_pos = Utf8Util::charPosToBytePos(text, start_char_pos);

    OnigRegion *region = onig_region_new();
    const OnigUChar *start = (const OnigUChar *)(text.c_str() + start_byte_pos);
    const OnigUChar *end = (const OnigUChar *)(text.c_str() + text.length());
    const OnigUChar *range_end = end;

    int match_byte_pos = onig_search(state_rule.regex, (OnigUChar *)text.c_str(),
                                     end, start, range_end, region, ONIG_OPTION_NONE);
    if (match_byte_pos >= 0)
    {
      size_t match_start_byte = match_byte_pos;
      size_t match_end_byte = region->end[0];
      if (match_end_byte <= match_start_byte)
      {
        onig_region_free(region, 1);
        return result;
      }
      size_t match_length_bytes = match_end_byte - match_start_byte;

      size_t match_start_char = Utf8Util::bytePosToCharPos(text, match_start_byte);
      size_t match_end_char = Utf8Util::bytePosToCharPos(text, match_end_byte);
      size_t match_length_chars = match_end_char - match_start_char;

      result.matched = true;
      result.start = match_start_char;
      result.length = match_length_chars;
      // result.state is not set here as we might not know the ID, but that's fine
      result.matched_text = Utf8Util::utf8Substr(text, match_start_char, match_length_chars);

      findMatchedRuleAndGroup(state_rule, region, text, match_start_byte, match_end_byte, result);

      // Subpatterns
      if (result.matched && result.token_rule_idx >= 0)
      {
        const TokenRule &rule = state_rule.token_rules[result.token_rule_idx];
        if (rule.sub_state_rule)
        {
          size_t sub_pos = 0;
          size_t sub_len = Utf8Util::countChars(result.matched_text);
          while (sub_pos < sub_len)
          {
            MatchResult sub_res = matchAtPosition(result.matched_text, sub_pos, *rule.sub_state_rule);
            if (!sub_res.matched)
            {
              break;
            }

            // Gap
            if (sub_res.start > sub_pos)
            {
              TokenSpan gap;
              gap.range.start.column = sub_pos;
              gap.range.end.column = sub_res.start;
              gap.style_id = result.style;
              if (m_config_.inline_style)
                gap.inline_style = m_rule_->getInlineStyle(result.style);
              result.sub_spans.push_back(gap);
            }

            // Sub-match content
            if (!sub_res.sub_spans.empty())
            {
              for (auto &s : sub_res.sub_spans)
              {
                s.range.start.column += sub_res.start;
                s.range.end.column += sub_res.start;
                result.sub_spans.push_back(s);
              }
            }
            else if (!sub_res.capture_groups.empty())
            {
              for (auto &g : sub_res.capture_groups)
              {
                TokenSpan s;
                s.range.start.column = g.start + sub_res.start;
                s.range.end.column = g.start + g.length + sub_res.start;
                s.style_id = g.style;
                if (m_config_.inline_style)
                  s.inline_style = m_rule_->getInlineStyle(g.style);
                result.sub_spans.push_back(s);
              }
            }
            else
            {
              TokenSpan s;
              s.range.start.column = sub_res.start;
              s.range.end.column = sub_res.start + sub_res.length;
              s.style_id = sub_res.style;
              if (m_config_.inline_style)
                s.inline_style = m_rule_->getInlineStyle(sub_res.style);
              result.sub_spans.push_back(s);
            }

            sub_pos = sub_res.start + sub_res.length;
            if (sub_res.length == 0)
              sub_pos++;
          }
          // Tail
          if (sub_pos < sub_len)
          {
            TokenSpan gap;
            gap.range.start.column = sub_pos;
            gap.range.end.column = sub_len;
            gap.style_id = result.style;
            if (m_config_.inline_style)
              gap.inline_style = m_rule_->getInlineStyle(result.style);
            result.sub_spans.push_back(gap);
          }
        }
      }
    }
    onig_region_free(region, 1);
    return result;
  }

  void LineHighlightAnalyzer::findMatchedRuleAndGroup(const StateRule &state_rule, const OnigRegion *region,
                                                      const U8String &text, size_t match_start_byte, size_t match_end_byte, MatchResult &result)
  {
    for (int32_t rule_idx = 0; rule_idx < static_cast<int32_t>(state_rule.token_rules.size()); ++rule_idx)
    {
      const TokenRule &token_rule = state_rule.token_rules[rule_idx];
      int32_t token_group_start = token_rule.group_offset_start;

      if (region->beg[token_group_start] == static_cast<int>(match_start_byte) && region->end[token_group_start] == static_cast<int>(match_end_byte))
      {
        result.token_rule_idx = rule_idx;
        result.goto_state = token_rule.goto_state;
        result.style = token_rule.getGroupStyleId(0);
        result.matched_group = token_group_start;

        for (int32_t group = 1; group <= token_rule.group_count; ++group)
        {
          int32_t absolute_group = group + token_group_start;
          int group_start_byte = region->beg[absolute_group];
          int group_end_byte = region->end[absolute_group];
          if (group_start_byte >= static_cast<int>(match_start_byte) && group_end_byte <= static_cast<int>(match_end_byte))
          {
            CaptureGroupMatch group_match;
            group_match.group = group;
            group_match.style = token_rule.getGroupStyleId(group);
            size_t match_start_char = Utf8Util::bytePosToCharPos(text, group_start_byte);
            size_t match_end_char = Utf8Util::bytePosToCharPos(text, group_end_byte);
            size_t match_length_chars = match_end_char - match_start_char;
            group_match.start = match_start_char;
            group_match.length = match_length_chars;
            result.capture_groups.push_back(group_match);
          }
        }
        return;
      }
    }
  }

  void LineHighlightAnalyzer::addLineHighlightResult(LineHighlight &highlight, const TextLineInfo &info,
                                                     int32_t syntax_state, const MatchResult &match_result) const
  {
    if (!match_result.sub_spans.empty())
    {
      for (const auto &sub : match_result.sub_spans)
      {
        TokenSpan span = sub;
        // Shift to absolute line column
        span.range.start.column += match_result.start;
        span.range.end.column += match_result.start;

        // Set line number
        span.range.start.line = info.line;
        span.range.end.line = info.line;

        // Set byte index
        span.range.start.index = info.start_char_offset + span.range.start.column;
        span.range.end.index = info.start_char_offset + span.range.end.column;

        span.state = syntax_state;
        highlight.pushOrMergeSpan(std::move(span));
      }
    }
    else if (match_result.capture_groups.empty())
    {
      TokenSpan span;
      span.range.start = {
          info.line,
          match_result.start,
          info.start_char_offset + match_result.start};
      span.range.end = {
          info.line,
          match_result.start + match_result.length,
          info.start_char_offset + match_result.start + match_result.length};
      span.state = syntax_state;
      span.matched_text = match_result.matched_text;
      span.style_id = match_result.style;
      if (m_config_.inline_style)
      {
        span.inline_style = m_rule_->getInlineStyle(match_result.style);
      }
      span.goto_state = match_result.goto_state;
      highlight.pushOrMergeSpan(std::move(span));
    }
    else
    {
      for (const CaptureGroupMatch &group_match : match_result.capture_groups)
      {
        TokenSpan span;
        span.range.start = {
            info.line,
            group_match.start,
            info.start_char_offset + group_match.start};
        span.range.end = {
            info.line,
            group_match.start + group_match.length,
            info.start_char_offset + group_match.start + group_match.length};
        span.state = syntax_state;
        span.style_id = group_match.style;
        if (m_config_.inline_style)
        {
          span.inline_style = m_rule_->getInlineStyle(group_match.style);
        }
        span.goto_state = match_result.goto_state;
        highlight.pushOrMergeSpan(std::move(span));
      }
    }
  }

  // ===================================== InternalDocumentAnalyzer ============================================
  InternalDocumentAnalyzer::InternalDocumentAnalyzer(const SharedPtr<Document> &document, const SharedPtr<SyntaxRule> &rule,
                                                     const HighlightConfig &config) : m_document_(document), m_rule_(rule), m_config_(config)
  {
    m_highlight_ = makeSharedPtr<DocumentHighlight>();
    m_line_highlight_analyzer_ = makeUniquePtr<LineHighlightAnalyzer>(m_rule_, config);
  }

  SharedPtr<DocumentHighlight> InternalDocumentAnalyzer::analyzeHighlight()
  {
    if (m_rule_ == nullptr)
    {
      return nullptr;
    }
    int32_t current_state = SyntaxRule::kDefaultStateId;
    const size_t line_count = m_document_->getLineCount();
    m_line_syntax_states_.resize(line_count, {});
    m_highlight_->reset();
    size_t line_start_index = 0;
    for (size_t line_num = 0; line_num < line_count; ++line_num)
    {
      TextLineInfo info = {line_num, current_state, line_start_index};
      LineAnalyzeResult result;
      const DocumentLine &document_line = m_document_->getLine(line_num);
      m_line_highlight_analyzer_->analyzeLine(document_line.text, info, result);
      m_line_syntax_states_[line_num] = result.end_state;
      m_highlight_->addLine(std::move(result.highlight));
      current_state = result.end_state;
      line_start_index += result.char_count + Document::getLineEndingWidth(m_document_->getLine(line_num).ending);
    }
    return m_highlight_;
  }

  SharedPtr<DocumentHighlight> InternalDocumentAnalyzer::analyzeHighlightIncremental(const TextRange &range, const U8String &new_text)
  {
    if (m_rule_ == nullptr)
    {
      return nullptr;
    }
    int32_t line_change = m_document_->patch(range, new_text);
    size_t change_start_line = range.start.line;
    size_t change_end_line = static_cast<int32_t>(range.end.line) + line_change;
    // m_line_syntax_states_[change_start_line] = change_start_line > 0 ? m_line_syntax_states_[change_start_line - 1] : SyntaxRule::kDefaultStateId;
    if (line_change < 0)
    {
      m_line_syntax_states_.erase(m_line_syntax_states_.begin() + range.end.line + line_change + 1,
                                  m_line_syntax_states_.begin() + range.end.line + 1);
      m_highlight_->lines.erase(m_highlight_->lines.begin() + range.end.line + line_change + 1,
                                m_highlight_->lines.begin() + range.end.line + 1);
    }
    else if (line_change > 0)
    {
      m_line_syntax_states_.insert(m_line_syntax_states_.begin() + range.end.line + 1, line_change, {});
      m_highlight_->lines.insert(m_highlight_->lines.begin() + range.end.line + 1, line_change, {});
    }

    // 从patch的起始行开始分析，直到状态稳定
    int32_t current_state = change_start_line > 0 ? m_line_syntax_states_[change_start_line - 1] : SyntaxRule::kDefaultStateId;
    size_t total_line_count = m_document_->getLineCount();
    size_t line_start_index = m_document_->charIndexOfLine(change_start_line);
    size_t line = change_start_line;
    bool stable = false;
    for (; line < total_line_count; ++line)
    {
      if (stable)
      {
        break;
      }
      int32_t old_state = m_line_syntax_states_[line];
      TextLineInfo line_info = {line, current_state, line_start_index};
      LineAnalyzeResult result;
      const DocumentLine &document_line = m_document_->getLine(line);
      m_line_highlight_analyzer_->analyzeLine(document_line.text, line_info, result);
      m_line_syntax_states_[line] = result.end_state;
      current_state = result.end_state;

      // 已经将patch range末尾后一行分析完毕，检查状态是否已经稳定
      if (line > change_end_line && old_state == current_state)
      {
        /*stable = true;
        for (size_t check_line = line + 1; check_line < total_line_count; ++check_line) {
          if (line_states_[check_line] != highlight_->lines[check_line].spans.back().state) {
            stable = false;
            break;
          }
        }*/
        // 或许不需要遍历后续所有行比对状态？只需要这一行的状态和高亮信息与patch前一致就可以判定为稳定
        const LineHighlight &old_line_highlight = m_highlight_->lines[line];
        if (old_line_highlight == result.highlight)
        {
          stable = true;
        }
      }
      m_highlight_->lines[line] = std::move(result.highlight);
      line_start_index += result.char_count + Document::getLineEndingWidth(m_document_->getLine(line).ending);
    }
    // 更新后续行的索引
    if (m_config_.show_index)
    {
      for (; line < total_line_count; ++line)
      {
        LineHighlight &line_highlight = m_highlight_->lines[line];
        for (TokenSpan &span : line_highlight.spans)
        {
          span.range.start.index = line_start_index + span.range.start.column;
          span.range.end.index = line_start_index + span.range.end.column;
        }
        line_start_index += m_document_->getLineCharCount(line);
      }
    }
    return m_highlight_;
  }

  SharedPtr<DocumentHighlight> InternalDocumentAnalyzer::analyzeHighlightIncremental(size_t start_index, size_t end_index, const U8String &new_text)
  {
    TextPosition start_pos = m_document_->charIndexToPosition(start_index);
    end_index = std::min(end_index, m_document_->totalChars());
    TextPosition end_pos = m_document_->charIndexToPosition(end_index);
    return analyzeHighlightIncremental(TextRange{start_pos, end_pos}, new_text);
  }

  SharedPtr<Document> InternalDocumentAnalyzer::getDocument() const
  {
    return m_document_;
  }

  const HighlightConfig &InternalDocumentAnalyzer::getHighlightConfig() const
  {
    return m_config_;
  }

  // ===================================== DocumentAnalyzer ============================================
  DocumentAnalyzer::DocumentAnalyzer(const SharedPtr<Document> &document, const SharedPtr<SyntaxRule> &rule,
                                     const HighlightConfig &config) : analyzer_impl_(makeUniquePtr<InternalDocumentAnalyzer>(document, rule, config))
  {
  }

  SharedPtr<DocumentHighlight> DocumentAnalyzer::analyze() const
  {
    return analyzer_impl_->analyzeHighlight();
  }

  SharedPtr<DocumentHighlight> DocumentAnalyzer::analyzeIncremental(const TextRange &range, const U8String &new_text) const
  {
    return analyzer_impl_->analyzeHighlightIncremental(range, new_text);
  }

  SharedPtr<DocumentHighlight> DocumentAnalyzer::analyzeIncremental(size_t start_index, size_t end_index, const U8String &new_text) const
  {
    return analyzer_impl_->analyzeHighlightIncremental(start_index, end_index, new_text);
  }

  SharedPtr<Document> DocumentAnalyzer::getDocument() const
  {
    return analyzer_impl_->getDocument();
  }

  const HighlightConfig &DocumentAnalyzer::getHighlightConfig() const
  {
    return analyzer_impl_->getHighlightConfig();
  }

  // ===================================== HighlightEngine ============================================
  HighlightEngine::HighlightEngine(const HighlightConfig &config) : m_config_(config)
  {
    m_style_mapping_ = makeSharedPtr<StyleMapping>();
  }

  SharedPtr<SyntaxRule> HighlightEngine::compileSyntaxFromJson(const U8String &json)
  {
    auto provider = [this](const U8String &name)
    { return this->getSyntaxRuleByName(name); };
    UniquePtr<SyntaxRuleCompiler> compiler = makeUniquePtr<SyntaxRuleCompiler>(m_style_mapping_, m_config_.inline_style, provider);
    SharedPtr<SyntaxRule> rule = compiler->compileSyntaxFromJson(json);
    m_syntax_rules_.emplace(rule);
    return rule;
  }

  SharedPtr<SyntaxRule> HighlightEngine::compileSyntaxFromFile(const U8String &file)
  {
    auto provider = [this](const U8String &name)
    { return this->getSyntaxRuleByName(name); };
    UniquePtr<SyntaxRuleCompiler> compiler = makeUniquePtr<SyntaxRuleCompiler>(m_style_mapping_, m_config_.inline_style, provider);
    SharedPtr<SyntaxRule> rule = compiler->compileSyntaxFromFile(file);
    m_syntax_rules_.emplace(rule);
    return rule;
  }

  SharedPtr<SyntaxRule> HighlightEngine::getSyntaxRuleByName(const U8String &name) const
  {
    for (const SharedPtr<SyntaxRule> &rule : m_syntax_rules_)
    {
      if (rule->name == name)
      {
        return rule;
      }
    }
    return nullptr;
  }

  SharedPtr<SyntaxRule> HighlightEngine::getSyntaxRuleByExtension(const U8String &extension) const
  {
    if (extension.empty())
    {
      return nullptr;
    }
    U8String fixed_extension = extension;
    if (fixed_extension[0] != '.')
    {
      fixed_extension.insert(0, ".");
    }
    for (const SharedPtr<SyntaxRule> &rule : m_syntax_rules_)
    {
      if (rule->file_extensions.find(fixed_extension) != rule->file_extensions.end())
      {
        return rule;
      }
    }
    return nullptr;
  }

  void HighlightEngine::registerStyleName(const U8String &style_name, int32_t style_id) const
  {
    m_style_mapping_->registerStyleName(style_name, style_id);
  }

  const U8String &HighlightEngine::getStyleName(int32_t style_id) const
  {
    return m_style_mapping_->getStyleName(style_id);
  }

  SharedPtr<TextAnalyzer> HighlightEngine::createAnalyzerByName(const U8String &syntax_name) const
  {
    SharedPtr<SyntaxRule> rule = getSyntaxRuleByName(syntax_name);
    if (rule == nullptr)
    {
      return nullptr;
    }
    return makeSharedPtr<TextAnalyzer>(rule, m_config_);
  }

  SharedPtr<TextAnalyzer> HighlightEngine::createAnalyzerByExtension(const U8String &extension) const
  {
    SharedPtr<SyntaxRule> rule = getSyntaxRuleByExtension(extension);
    if (rule == nullptr)
    {
      return nullptr;
    }
    return makeSharedPtr<TextAnalyzer>(rule, m_config_);
  }

  SharedPtr<DocumentAnalyzer> HighlightEngine::loadDocument(const SharedPtr<Document> &document)
  {
    auto it = m_analyzer_map_.find(document->getUri());
    if (it == m_analyzer_map_.end())
    {
      U8String uri = document->getUri();
      SharedPtr<SyntaxRule> rule = getSyntaxRuleByExtension(FileUtil::getExtension(uri));
      if (rule == nullptr)
      {
        return nullptr;
      }
      SharedPtr<DocumentAnalyzer> analyzer = SharedPtr<DocumentAnalyzer>(new DocumentAnalyzer(document, rule, m_config_));
      m_analyzer_map_.insert_or_assign(uri, analyzer);
      return analyzer;
    }
    else
    {
      return it->second;
    }
  }

  void HighlightEngine::removeDocument(const U8String &uri)
  {
    m_analyzer_map_.erase(uri);
  }
}
