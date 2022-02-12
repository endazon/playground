// DebuggingConsole.cpp : アプリケーションのエントリ ポイントを定義します。
//

#include "DebuggingConsole.h"

using namespace Simulator;
int main()
{
	auto timer = []()
	{
		Signal<int> time(1, "Test", "テスト", "☆★☆彡");
		// QueryPerformanceCounter関数の1秒当たりのカウント数を取得する
		LARGE_INTEGER freq;
		QueryPerformanceFrequency(&freq);

		LARGE_INTEGER start, end;

		while (true)
		{
			{
				Signal<int> obj(time, std::string("Test") + time.to_string(), "テスト", "☆★☆彡");
				//GeneralPurposeTimer::Measurement::MeasuringElapsedTime<GeneralPurposeTimer::Measurement::LowPrecision>    elapsedTime1;
				//GeneralPurposeTimer::Measurement::MeasuringElapsedTime<GeneralPurposeTimer::Measurement::MediumPrecision> elapsedTime2;
				//GeneralPurposeTimer::Measurement::MeasuringElapsedTime<GeneralPurposeTimer::Measurement::HighPrecision>   elapsedTime3;
				QueryPerformanceCounter(&start);
				QueryPerformanceCounter(&end);
				for (; static_cast<double>(end.QuadPart - start.QuadPart) * 1000.0 / freq.QuadPart < 200; QueryPerformanceCounter(&end));
				time++;
			}
		}

	};

	std::thread thread(timer);
	thread.join();

	return 0;
}
