
#pragma once

#include <iostream>
#include <stdint.h>
#include <type_traits>
#include <cassert>
#include <time.h>
#include <chrono>
#include <windows.h>
#include <sstream>
#include <regex>

namespace GeneralPurposeTimer
{
	namespace Accessor
	{
		template<class __InheritanceType>
		class timeOperation : public __InheritanceType
		{
		public:
			using __InheritanceType::__InheritanceType;

			inline void print(__InheritanceType::rep count) { std::cout << count << this->units2Display() << std::endl; }
			inline void print()								{ print(this->count()); }
		};

		class nanosecondsOperation : public std::chrono::nanoseconds
		{
		public:
			using std::chrono::nanoseconds::nanoseconds;
			constexpr nanosecondsOperation(const std::chrono::nanoseconds& t) noexcept : std::chrono::nanoseconds(t) {}

			constexpr std::string units2Display() { return "[ ns]"; }
		};
		class microsecondsOperation : public std::chrono::microseconds
		{
		public:
			using std::chrono::microseconds::microseconds;
			constexpr microsecondsOperation(const std::chrono::microseconds& t) noexcept : std::chrono::microseconds(t) {}

			constexpr std::string units2Display() { return "[us]"; }
		};
		class millisecondsOperation : public std::chrono::milliseconds
		{
		public:
			using std::chrono::milliseconds::milliseconds;
			constexpr millisecondsOperation(const std::chrono::milliseconds& t) noexcept : std::chrono::milliseconds(t) {}

			constexpr std::string units2Display() { return "[ms]"; }
		};
		class secondsOperation : public std::chrono::seconds
		{
		public:
			using std::chrono::seconds::seconds;
			constexpr secondsOperation(const std::chrono::seconds& t) noexcept : std::chrono::seconds(t) {}

			constexpr std::string units2Display() { return "[s]"; }
		};
		class minutesOperation : public std::chrono::minutes
		{
		public:
			using std::chrono::minutes::minutes;
			constexpr minutesOperation(const std::chrono::minutes& t) noexcept : std::chrono::minutes(t) {}

			constexpr std::string units2Display() { return "[min]"; }
		};
		class hoursOperation : public std::chrono::hours
		{
		public:
			using std::chrono::hours::hours;
			constexpr hoursOperation(const std::chrono::hours& t) noexcept : std::chrono::hours(t) {}

			constexpr std::string units2Display() { return "[h]"; }
		};
		class daysOperation : public std::chrono::days
		{
		public:
			using std::chrono::days::days;
			constexpr daysOperation(const std::chrono::days& t) noexcept : std::chrono::days(t) {}

			constexpr std::string units2Display() { return "[d]"; }
		};
		class weeksOperation : public std::chrono::weeks
		{
		public:
			using std::chrono::weeks::weeks;
			constexpr weeksOperation(const std::chrono::weeks& t) noexcept : std::chrono::weeks(t) {}

			constexpr std::string units2Display() { return "[w]"; }
		};
		class monthsOperation : public std::chrono::months
		{
		public:
			using std::chrono::months::months;
			constexpr monthsOperation(const std::chrono::months& t) noexcept : std::chrono::months(t) {}

			constexpr std::string units2Display() { return "[m]"; }
		};
		class yearsOperation : public std::chrono::years
		{
		public:
			using std::chrono::years::years;
			constexpr yearsOperation(const std::chrono::years& t) noexcept : std::chrono::years(t) {}

			constexpr std::string units2Display() { return "[y]"; }
		};

		using nanoseconds	= timeOperation<nanosecondsOperation>;
		using microseconds	= timeOperation<microsecondsOperation>;
		using milliseconds	= timeOperation<millisecondsOperation>;
		using seconds		= timeOperation<secondsOperation>;
		using minutes		= timeOperation<minutesOperation>;
		using hours			= timeOperation<hoursOperation>;
		using days			= timeOperation<daysOperation>;
		using weeks			= timeOperation<weeksOperation>;
		using months		= timeOperation<monthsOperation>;
		using years			= timeOperation<yearsOperation>;

		//参考にしたサイト
		//https://memo.appri.me/programming/cpp-date-time
		class dateFormat
		{
		private:
			/**
			 * Check whether ISO date string with milliseconds or not.
			 * @param  isoStr {string} A ISO date string. e.g. "2021-02-09T01:46:45.595Z".
			 * @return {bool} e.g. Passing "2021-02-09T01:46:45.595Z" (has millisecond's part) returns true.
			 */
			bool isLongISOString(std::string isoStr) 
			{
				std::smatch m;
				std::regex_match(isoStr, m, std::regex(R"(^(\d+)-(\d+)-(\d+)T(\d+):(\d+):(\d+)\.(\d+)Z$)"));
				if (m.size() >= 1) { return true; }
				return false;
			}

		public:
			/**
			 * Get a current time in millisecond.
			 * @return {long int} a current unix epoch time in millisecond.
			 */
			inline unsigned long long getTime()
			{
				return std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch()).count();
			}
			/**
			 * Converts an ISO date string to unix epoch time and returns it.
			 * @param  isoStr {string} A ISO date string. e.g. "2021-02-09T01:46:45.595Z"
			 * @return {long int} An unix epoch time in milliseconds.
			 */
			long long getTime(std::string isoStr)
			{
				struct tm : public std::tm
				{
					int tm_msec;
				};

				auto timeConversion = [this](std::string isoStr)
				{
					tm tm = {};
					if (isLongISOString(isoStr))
					{
						// with milliseconds' part:
						assert(sscanf(isoStr.c_str(), "%d-%d-%dT%d:%d:%d.%dZ", &tm.tm_year, &tm.tm_mon, &tm.tm_mday, &tm.tm_hour, &tm.tm_min, &tm.tm_sec, &tm.tm_msec) == 7);
					}
					else
					{
						// without milliseconds' part:
						assert(sscanf(isoStr.c_str(), "%d-%d-%dT%d:%d:%dZ", &tm.tm_year, &tm.tm_mon, &tm.tm_mday, &tm.tm_hour, &tm.tm_min, &tm.tm_sec) == 6);
					}

					tm.tm_hour += 9;
					tm.tm_mon -=1;
					tm.tm_year -=1900;
					tm.tm_isdst = -1; // Use DST value from local time zone

					return tm;
				};

				tm tm = timeConversion(isoStr);
				auto msec = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::from_time_t(std::mktime(&tm)).time_since_epoch()).count();
				return msec + tm.tm_msec; // Add actual millisecond's value to the rough result in millisecond.
			}

			/**
			 * Get an ISO date string.
			 * @param  msec {long int}
			 * @return {std::string} An ISO date string. e.g. "2021-02-09T01:46:45.595Z".
			 */
			std::string getISOString(unsigned long long msec)
			{
				time_t sec = msec / 1000;
				char isoStr[sizeof("2021-01-31T23:59:59.000Z")];
				strftime(isoStr, sizeof(isoStr), "%FT%T", gmtime(&sec));
				int delta = msec - (sec * 1000);
				sprintf(isoStr, "%s.%03dZ", isoStr, delta);
				return isoStr;
			}
			/**
			 * Get a current ISO date string.
			 * @return {std::string} An ISO date string. e.g. "2021-02-09T01:46:45.595Z".
			 */
			inline std::string getISOString()
			{
				return getISOString(getTime());
			}
			inline std::string getISOStringForJST()
			{
				return getISOString(getTime() + 9 * 60 * 60 * 1000);
			}
			
			/**
			 * Get a local date string.
			 * @param msec {long int} An unix epoch time in milliseconds.
			 * @param dateformat {char*} A date format. e.g. "%Y/%m/%d(%a)%H:%M:%S"
			 * @return {std::string} A local date string. e.g. "2021/02/09(Tue)16:20:30".
			 */
			std::string getLocaleString(unsigned long long msec, const char* dateformat) {
				time_t sec = msec / 1000;
				struct tm* timeinfo;
				timeinfo = localtime(&sec);
				const int charsize = sizeof "2021/02/09(Tue)23:59:59";
				char output[charsize];
				strftime(output, charsize, dateformat, timeinfo);
				return std::string(output);
			}
			/**
			 * Get a local date string.
			 * @param msec {long int} An unix epoch time in milliseconds.
			 * @return {std::string} A local date string. e.g. "2021/02/09(Tue)16:20:30".
			 */
			inline std::string getLocaleString(unsigned long long msec)
			{
				return getLocaleString(msec, "%Y/%m/%d(%a)%H:%M:%S");
			}
			/**
			 * Get a current local date string.
			 * @return {std::string} A local date string. e.g. "2021/02/09(Tue)16:20:30".
			 */
			inline std::string getLocaleString() {
				return getLocaleString(getTime());
			}
		};
	}

	//参考にしたサイト
	//http://www.sanko-shoko.net/note.php?id=rnfd
	namespace Measurement
	{
		//C/C++で処理時間の計測を行う時，time.h で定義されている clock()関数が良く利用されます．
		//ただ，clock()関数の分解能は10[ms]程度ですので，短い処理の計測には向きません．
		class LowPrecision
		{
		public:
			using __CountType = Accessor::milliseconds;

			inline __CountType getTime()
			{
				return __CountType(clock());
			}
		};

		//<chrono>で定義されているクラスで，1[ms]程度の分解能で時間計測が可能です．
		//C++11をコンパイルできる環境があれば利用できるため，クロスプラットフォームを考える場合は有用だと思います．
		class MediumPrecision
		{
		public:
			using __CountType = Accessor::microseconds;

			inline __CountType getTime()
			{
				return __CountType(std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::system_clock::now().time_since_epoch()));
			}
		};

		//windows.hで定義されている関数で，1[ms]以下の細かい分解能で時間計測が可能です．
		//clock関数に比べると使用法がやや複雑ですが，精度は高いためwindows環境であれば採用を検討しても良いと思います．
		class HighPrecision
		{
		private:
			size_t DigitAdjustment = 1;

		public:
			using __CountType = Accessor::nanoseconds;

			HighPrecision()
			{
				LARGE_INTEGER time;
				QueryPerformanceFrequency(&time);
				for (size_t i = 0; i < (static_cast<int>(log10(time.QuadPart)) + 1) % 3; i++) { DigitAdjustment *= 10; }
			}

			inline __CountType getTime()
			{
				LARGE_INTEGER time;
				QueryPerformanceCounter(&time);
				return __CountType(time.QuadPart * DigitAdjustment);
			}
		};

		template<class __PrecisionType>
		class MeasuringElapsedTime
		{
			static_assert(
				std::is_same<__PrecisionType, LowPrecision>::value    ||
				std::is_same<__PrecisionType, MediumPrecision>::value ||
				std::is_same<__PrecisionType, HighPrecision>::value   ,
				"type error"
				);

		private:
			using __CountType = __PrecisionType::__CountType;

			__PrecisionType timerAccessor;
			__CountType startTime;
			__CountType endTime;

		protected:
			inline void setStartTine(){ startTime = timerAccessor.getTime(); }
			inline void setEndTine()  { endTime   = timerAccessor.getTime(); }
			inline void print()
			{
				__CountType elapsedTime = endTime - startTime;
				elapsedTime.print();
			}

		public:
			inline MeasuringElapsedTime() noexcept  { setStartTine();          }
			inline ~MeasuringElapsedTime() noexcept { setEndTine();   print(); }
		};
	}
}