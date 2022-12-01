#pragma once

#include <ns3/vector.h>
#include <ns3/ipv4-address.h>

void SetColor(const ns3::Ipv4Address& address, ns3::Vector color);

bool ShouldDoCyberAttack();

struct SimulationParameters
{
  double a = 1.0;
  double r = 1.0;

  uint64_t seed = 0;
  uint32_t peripheralNodes = 7;
  double spawnRadius = 4;
  double duration = 180;
  double packetInterval = 1.5;
  double calculateInterval = 0.01;
  std::string positionsFile = "positions.csv";
};

extern SimulationParameters s_Parameters;

