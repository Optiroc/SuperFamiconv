#ifndef OPTIONS_H
#define OPTIONS_H


#ifdef _MSC_VER // visual c++
  #include <io.h>
  #include <winsock.h>
  #include "getopt-win/getopt.h"
#else
  #include <unistd.h>
  #include <getopt.h>
  #ifdef _WIN32 // mingw/cygwin
    #include <winsock.h>
  #else // everything else
    #include <sys/ioctl.h>
  #endif
#endif

#include <cstring>
#include <functional>
#include <iostream>
#include <map>
#include <set>
#include <sstream>
#include <stdexcept>
#include <string>
#include <type_traits>
#include <vector>

class Options {
public:
    Options() : IndentFlag(2), IndentDescription(18), optval(256) {}

    template <typename T>
    void Add(T& var, char flag, std::string long_flag, std::string description = "", T default_val = T(), std::string group = "");
    void AddSwitch(bool& var, char flag, std::string long_flag, std::string description = "", bool default_val = false, std::string group = "");

    bool Parse(int argc, char **argv);
    bool WasSet(const std::string& long_flag);

    std::string Usage();
    std::string Header;
    unsigned IndentFlag;
    unsigned IndentDescription;

private:
    std::set<std::string> long_flags;
    std::vector<struct option> options;
    std::map<int, std::function<void(std::string)>> setters;
    std::map<std::string, std::vector<std::string>> usage;
    std::set<int> was_set;

    int optval;
    std::string optstr;

    template <typename T>
    void set(T& var, std::string optarg);

    template <typename T>
    void add_entry(struct option& op, char flag, std::string long_flag, std::string description, std::string group, bool req_arg = false);

    template <typename T>
    std::string get_default(T& default_val);

    int tty_width();
};

template <typename T>
inline void Options::Add(T& var, char flag, std::string long_flag, std::string description, T default_val, std::string group) {
    struct option op;
    this->add_entry<T&>(op, flag, long_flag, description, group, true);

    op.has_arg = required_argument;
    var = default_val;

    this->setters[op.val] = std::bind(&Options::set<T>, this, std::ref(var), std::placeholders::_1);
    this->options.push_back(op);
}

inline void Options::AddSwitch(bool& var, char flag, std::string long_flag, std::string description, bool default_val, std::string group) {
    struct option op;
    this->add_entry<bool&>(op, flag, long_flag, description, group);

    op.has_arg = optional_argument;
    var = default_val;

    this->setters[op.val] = [&var](std::string) {
        var = !var;
    };

    this->options.push_back(op);
}

inline bool Options::Parse(int argc, char** argv) {
    this->options.push_back({0, 0, 0, 0});
    int ch;
    while ((ch = getopt_long(argc, argv, this->optstr.c_str(), &this->options[0], NULL)) != -1) {
        auto it = this->setters.find(ch);
        if (it != this->setters.end()) {
            this->was_set.insert(ch);
            it->second(optarg ? optarg : "");
        } else {
            return false;
        }
    }
    return true;
}

inline bool Options::WasSet(const std::string& long_flag) {
    for (auto opt : this->options) {
      if (strcmp(opt.name, long_flag.c_str()) == 0) {
        return (this->was_set.find(opt.val) != this->was_set.end());
      }
    }
    return false;
}

inline std::string Options::Usage() {
    std::stringstream ss;
    if (Header.size()) ss << Header;

    for (auto &it : this->usage) {
        if (it.first.size() && it.first.compare(std::string("_"))) ss << it.first << ":\n";
        for (auto& description : it.second) ss << description << '\n';
        ss << '\n';
    }
    return ss.str();
}


template <typename T>
inline void Options::add_entry(struct option& opt, char flag, std::string long_flag, std::string description, std::string group, bool req_arg) {
    if (!flag && !long_flag.size()) return;

    if (flag) {
        if (this->setters.find((int)(flag)) != this->setters.end()) {
            std::stringstream ss;
            ss << "Duplicate flag '" << flag << "'";
            throw std::runtime_error(ss.str());
        }
        this->optstr += flag;
        if (req_arg) this->optstr += ":";

        opt.val = flag;
    } else {
        opt.val = this->optval++;
    }

    if (long_flag.size()) {
        if (this->long_flags.find(long_flag) != this->long_flags.end()) {
            std::stringstream ss;
            ss << "Duplicate long flag \"" << long_flag << "\"";
            throw std::runtime_error(ss.str());
        }
        opt.name = this->long_flags.insert(long_flag).first->c_str();
    }
    opt.flag = NULL;

    if (description.size()) {
        std::stringstream ss;
        ss << std::string(IndentFlag, ' ');

        if (flag) ss << "-" << flag << " ";
        if (long_flag.size()) ss << "--" << long_flag << " ";
        ss << std::string(ss.str().length() > IndentDescription ? 1 : IndentDescription - ss.str().length(), ' ');

        std::string desc(description);
        if (std::is_same<T, bool&>::value) desc.append(std::string(" <switch>"));

        unsigned width = tty_width();
        unsigned desc_pos = width - ss.str().length() > width * 0.3f ? (unsigned)ss.str().length() : IndentFlag + 2;
        unsigned column_width = width - desc_pos;
        if (desc_pos == IndentFlag + 2) ss << '\n';

        for (unsigned i = 0; i < desc.size(); i += column_width) {
            while (desc.substr(i, 1) == std::string(" ")) ++i;
            if (i + column_width < desc.size()) {
                ss << desc.substr(i, column_width) << '\n' << std::string(desc_pos, ' ');
            } else {
                ss << desc.substr(i);
            }
        }

        this->usage[group].push_back(ss.str());
    }
}

template <typename T>
inline void Options::set(T& var, std::string opt_arg) {
    std::stringstream ss(opt_arg);
    ss >> var;
}

template <>
inline void Options::set<std::string>(std::string& var, std::string opt_arg) {
    var = opt_arg;
}

inline int Options::tty_width() {
#ifdef TIOCGSIZE
    struct ttysize ts;
    ioctl(STDIN_FILENO, TIOCGSIZE, &ts);
    return ts.ts_cols >= 40 ? ts.ts_cols : 80;
#elif defined(TIOCGWINSZ)
    struct winsize ts;
    ioctl(STDIN_FILENO, TIOCGWINSZ, &ts);
    return ts.ws_col >= 40 ? ts.ws_col : 80;
#else
	return 80;
#endif
}

#endif //OPTIONS_H
