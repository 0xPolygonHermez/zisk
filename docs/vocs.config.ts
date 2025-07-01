import { defineConfig } from "vocs";

export default defineConfig({
  title: "ZisK",
  description: "High-performance, low-latency zkVM for Zero-Knowledge Proof Generation",
  logoUrl: "/zisk_logo.jpg",
  iconUrl: "/zisk_logo.jpg",
  sidebar: [
    {
      text: "Developers",
      items: [
        {
          text: "Installation",
          link: "/developers/installation",
        },
        {
          text: "Quickstart",
          link: "/developers/quickstart",
        },
        {
          text: "Writing Programs",
          link: "/developers/writing-programs",
        },
        {
          text: "Precompile",
          link: "/developers/precompile",
        },
      ],
    },
    {
      text: "Examples",
      items: [
        {
          text: "Fibonacci",
          link: "/examples/fibonacci",
        },
        {
          text: "Keccak",
          link: "/examples/keccak",
        },
        {
          text: "Ethereum Block Execution",
          link: "/examples/ethereum-block-exec",
        },
      ],
    },
    {
      text: "Protocols",
      items: [
        {
          text: "Overview",
          link: "/protocols/overview",
        },
        {
          text: "Execution",
          items: [
            {
              text: "Overview",
              link: "/protocols/execution/overview",
            },
          ],
        },
        {
          text: "Witness Generation",
          items: [
            {
              text: "Overview",
              link: "/protocols/witness-generation/overview",
            },
          ],
        },
        {
          text: "AIRS building",
          items: [
            {
              text: "Overview",
              link: "/protocols/airs-building/overview",
            },
          ],
        },
        {
          text: "Final Aggregation",
          items: [
            {
              text: "Overview",
              link: "/protocols/final-aggregation/overview",
            },
          ],
        },
      ],
    },
  ],
  topNav: [
    { 
      text: 'Developers',
      link: '/developers/quickstart',
    },
    {
      text: 'Protocols',
      link: '/protocols/overview',
    },
    {
      text: 'docs.rs',
      link: 'https://docs.rs/zisk/latest/zisk/',
    },
  ],
  socials: [
    {
      icon: "github",
      link: "https://github.com/0xPolygonHermez/zisk",
    },
    {
      icon: "telegram",
      link: "https://t.me/ziskvm",
    },
    {
      icon: "x",
      link: "https://x.com/ziskvm",
    },
  ],
});
